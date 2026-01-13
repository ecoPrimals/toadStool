// Slice operation - Extract a slice from a tensor
// Supports multi-dimensional slicing with start, end, step

struct SliceParams {
    input_size: u32,
    output_size: u32,
    start: u32,
    end: u32,
    step: u32,
    axis_stride: u32,  // Stride along the slice axis
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: SliceParams;

@compute @workgroup_size(256)
fn slice_1d(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_idx = global_id.x;
    
    if (out_idx >= params.output_size) {
        return;
    }
    
    // Calculate input index
    let in_idx = params.start + out_idx * params.step;
    
    if (in_idx < params.input_size) {
        output[out_idx] = input[in_idx];
    }
}

@compute @workgroup_size(256)
fn slice_axis(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_idx = global_id.x;
    
    if (out_idx >= params.output_size) {
        return;
    }
    
    // Calculate which block and position within block
    let block = out_idx / params.axis_stride;
    let offset = out_idx % params.axis_stride;
    
    // Map to input index
    let slice_pos = offset / params.step;
    let in_offset = params.start + slice_pos * params.step;
    let in_idx = block * params.axis_stride + in_offset;
    
    if (in_idx < params.input_size) {
        output[out_idx] = input[in_idx];
    }
}
