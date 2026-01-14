// Slice operation - Extract a slice from a tensor
// Supports multi-dimensional slicing with start, end, step

struct SliceParams {
    start: u32,
    end: u32,
    stride: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: SliceParams;

@compute @workgroup_size(256)
fn slice_1d(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_idx = global_id.x;
    let output_size = params.end - params.start;
    
    if (out_idx >= output_size) {
        return;
    }
    
    // Calculate input index
    let in_idx = params.start + out_idx * params.stride;
    
    if (in_idx < params.end) {
        output[out_idx] = input[in_idx];
    }
}

@compute @workgroup_size(256)
fn slice_axis(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_idx = global_id.x;
    let output_size = params.end - params.start;
    
    if (out_idx >= output_size) {
        return;
    }
    
    // Simple axis slicing
    let in_idx = params.start + out_idx;
    output[out_idx] = input[in_idx];
}
