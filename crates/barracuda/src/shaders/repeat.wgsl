// Repeat - repeat tensor along an axis
// Simplified: repeat entire input N times

struct RepeatParams {
    repeats: u32,
    input_size: u32,
    _padding: vec2<u32>,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: RepeatParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.input_size * params.repeats) {
        return;
    }
    
    let src_idx = idx % params.input_size;
    output[idx] = input[src_idx];
}
